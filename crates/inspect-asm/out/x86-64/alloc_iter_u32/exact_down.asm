inspect_asm::alloc_iter_u32::exact_down:
	push r15
	push r14
	push rbx
	test rdx, rdx
	je .LBB0_2
	mov rax, rdx
	shr rax, 61
	jne .LBB0_8
	lea rbx, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r8, rax
	sub r8, qword ptr [rcx + 8]
	cmp rbx, r8
	ja .LBB0_7
	sub rax, rbx
	and rax, -4
	mov qword ptr [rcx], rax
.LBB0_0:
	add rbx, -4
	shr rbx, 2
	lea rdi, [rdx - 1]
	cmp rbx, rdi
	cmovae rbx, rdi
	xor ecx, ecx
	cmp rbx, 7
	jb .LBB0_3
	mov r9, rax
	sub r9, rsi
	mov r8, rsi
	cmp r9, 32
	jb .LBB0_4
	inc rbx
	movabs rcx, 9223372036854775800
	and rcx, rbx
	lea r8, [rsi + 4*rcx]
	xor r9d, r9d
.LBB0_1:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp rcx, r9
	jne .LBB0_1
	cmp rbx, rcx
	jne .LBB0_4
	jmp .LBB0_6
.LBB0_2:
	mov eax, 4
	xor ecx, ecx
	jmp .LBB0_6
.LBB0_3:
	mov r8, rsi
.LBB0_4:
	lea rdx, [rsi + 4*rdx]
	add r8, 4
.LBB0_5:
	mov rsi, rcx
	mov ecx, dword ptr [r8 - 4]
	mov dword ptr [rax + 4*rsi], ecx
	lea rcx, [rsi + 1]
	cmp rdi, rsi
	je .LBB0_6
	lea rsi, [r8 + 4]
	cmp r8, rdx
	mov r8, rsi
	jne .LBB0_5
.LBB0_6:
	mov rdx, rcx
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_7:
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov rdx, r15
	jmp .LBB0_0
.LBB0_8:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
