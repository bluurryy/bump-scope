inspect_asm::alloc_iter_u32::exact_up:
	push r14
	push rbx
	push rax
	mov rax, rdx
	shr rax, 61
	jne .LBB0_7
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi]
	mov rax, qword ptr [r8]
	mov r9, qword ptr [r8 + 8]
	add rax, 3
	and rax, -4
	sub r9, rax
	cmp rcx, r9
	ja .LBB0_5
	add rcx, rax
	mov qword ptr [r8], rcx
	test rdx, rdx
	je .LBB0_4
.LBB0_0:
	lea rcx, [rdx - 1]
	movabs r8, 4611686018427387903
	and r8, rcx
	cmp rdx, r8
	cmovb r8, rdx
	mov rcx, rsi
	mov rdi, rax
	cmp r8, 8
	jb .LBB0_2
	mov r9, rax
	sub r9, rsi
	mov rcx, rsi
	mov rdi, rax
	cmp r9, 31
	jbe .LBB0_2
	inc r8
	mov ecx, r8d
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	sub r8, rdi
	lea rcx, [rsi + 4*r8]
	lea rdi, [rax + 4*r8]
	xor r9d, r9d
.LBB0_1:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp r8, r9
	jne .LBB0_1
.LBB0_2:
	lea rsi, [rsi + 4*rdx]
	lea r8, [rax + 4*rdx]
.LBB0_3:
	cmp rcx, rsi
	je .LBB0_6
	mov r9d, dword ptr [rcx]
	add rcx, 4
	mov dword ptr [rdi], r9d
	add rdi, 4
	cmp rdi, r8
	jne .LBB0_3
.LBB0_4:
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB0_5:
	mov rbx, rsi
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r14
	test rdx, rdx
	jne .LBB0_0
	jmp .LBB0_4
.LBB0_6:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
.LBB0_7:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rdi, rax
	call _Unwind_Resume@PLT
