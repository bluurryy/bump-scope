inspect_asm::alloc_iter_u32::exact_down:
	push r14
	push rbx
	push rax
	mov rax, rdx
	shr rax, 61
	jne .LBB_3
	lea r8, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r9, rax
	sub r9, qword ptr [rcx + 8]
	cmp r8, r9
	ja .LBB_4
	sub rax, r8
	and rax, -4
	mov qword ptr [rcx], rax
	test rdx, rdx
	je .LBB_16
.LBB_6:
	lea rcx, [rdx - 1]
	movabs r8, 4611686018427387903
	and r8, rcx
	cmp rdx, r8
	cmovb r8, rdx
	mov rcx, rsi
	mov rdi, rax
	cmp r8, 8
	jb .LBB_8
	mov r9, rax
	sub r9, rsi
	mov rcx, rsi
	mov rdi, rax
	cmp r9, 31
	jbe .LBB_8
	inc r8
	mov ecx, r8d
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	sub r8, rdi
	lea rcx, [rsi + 4*r8]
	lea rdi, [rax + 4*r8]
	xor r9d, r9d
.LBB_13:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp r8, r9
	jne .LBB_13
.LBB_8:
	lea rsi, [rsi + 4*rdx]
	lea r8, [rax + 4*rdx]
.LBB_9:
	cmp rcx, rsi
	je .LBB_10
	mov r9d, dword ptr [rcx]
	add rcx, 4
	mov dword ptr [rdi], r9d
	add rdi, 4
	cmp rdi, r8
	jne .LBB_9
.LBB_16:
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_4:
	mov rbx, rsi
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<_,_,A>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r14
	test rdx, rdx
	jne .LBB_6
	jmp .LBB_16
.LBB_10:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
.LBB_3:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rdi, rax
	call _Unwind_Resume@PLT