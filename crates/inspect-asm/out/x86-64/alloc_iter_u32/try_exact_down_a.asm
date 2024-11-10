inspect_asm::alloc_iter_u32::try_exact_down_a:
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
	push r15
	push r14
	push rbx
	lea rbx, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r8, rax
	sub r8, qword ptr [rcx + 8]
	cmp rbx, r8
	ja .LBB0_8
	sub rax, rbx
	mov qword ptr [rcx], rax
	je .LBB0_8
.LBB0_2:
	add rbx, -4
	shr rbx, 2
	lea rdi, [rdx - 1]
	cmp rbx, rdi
	cmovae rbx, rdi
	xor ecx, ecx
	cmp rbx, 7
	jb .LBB0_4
	mov r9, rax
	sub r9, rsi
	mov r8, rsi
	cmp r9, 32
	jb .LBB0_5
	inc rbx
	movabs rcx, 9223372036854775800
	and rcx, rbx
	lea r8, [rsi + 4*rcx]
	xor r9d, r9d
.LBB0_3:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp rcx, r9
	jne .LBB0_3
	cmp rbx, rcx
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
	pop rbx
	pop r14
	pop r15
	mov rdx, rcx
	ret
.LBB0_8:
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov rdx, r15
	test rax, rax
	jne .LBB0_2
	xor eax, eax
	jmp .LBB0_7
