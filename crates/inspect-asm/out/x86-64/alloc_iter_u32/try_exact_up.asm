inspect_asm::alloc_iter_u32::try_exact_up:
	test rdx, rdx
	je .LBB0_0
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
	xor eax, eax
	mov rdx, rcx
	ret
.LBB0_0:
	mov eax, 4
	xor ecx, ecx
	mov rdx, rcx
	ret
.LBB0_1:
	push r14
	push rbx
	push rax
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi]
	mov rax, qword ptr [r8]
	mov r9, qword ptr [r8 + 8]
	add rax, 3
	and rax, -4
	sub r9, rax
	cmp rcx, r9
	ja .LBB0_8
	add rcx, rax
	mov qword ptr [r8], rcx
	test rax, rax
	je .LBB0_8
.LBB0_2:
	lea rdi, [rdx - 1]
	movabs r9, 4611686018427387903
	and r9, rdi
	cmp r9, rdi
	cmovae r9, rdi
	xor ecx, ecx
	cmp r9, 7
	jb .LBB0_4
	mov r10, rax
	sub r10, rsi
	mov r8, rsi
	cmp r10, 32
	jb .LBB0_5
	inc r9
	movabs rcx, 9223372036854775800
	and rcx, r9
	lea r8, [rsi + 4*rcx]
	xor r10d, r10d
.LBB0_3:
	movups xmm0, xmmword ptr [rsi + 4*r10]
	movups xmm1, xmmword ptr [rsi + 4*r10 + 16]
	movups xmmword ptr [rax + 4*r10], xmm0
	movups xmmword ptr [rax + 4*r10 + 16], xmm1
	add r10, 8
	cmp rcx, r10
	jne .LBB0_3
	cmp r9, rcx
	jne .LBB0_5
	jmp .LBB0_7
.LBB0_4:
	mov r8, rsi
.LBB0_5:
	lea rdx, [rsi + 4*rdx]
	add r8, 4
.LBB0_6:
	mov rsi, rcx
	mov ecx, dword ptr [r8 - 4]
	mov dword ptr [rax + 4*rsi], ecx
	lea rcx, [rsi + 1]
	cmp rdi, rsi
	je .LBB0_7
	lea rsi, [r8 + 4]
	cmp r8, rdx
	mov r8, rsi
	jne .LBB0_6
.LBB0_7:
	add rsp, 8
	pop rbx
	pop r14
	mov rdx, rcx
	ret
.LBB0_8:
	mov rbx, rsi
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r14
	test rax, rax
	jne .LBB0_2
	xor eax, eax
	jmp .LBB0_7
