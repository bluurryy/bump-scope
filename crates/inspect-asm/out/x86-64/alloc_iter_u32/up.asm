inspect_asm::alloc_iter_u32::up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB0_6
	mov rax, rdx
	shr rax, 61
	jne .LBB0_8
	mov rbx, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rsi, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub rsi, rax
	cmp r15, rsi
	ja .LBB0_7
	lea rsi, [rax + r15]
	mov qword ptr [rcx], rsi
.LBB0_0:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r14, rsp
	xor edx, edx
	jmp .LBB0_2
.LBB0_1:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB0_3
.LBB0_2:
	mov ebp, dword ptr [rbx + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_1
	mov rdi, r14
	call bump_scope::bump_vec::BumpVec<T,A>::generic_grow_amortized
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_1
.LBB0_3:
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 16]
	mov rdi, qword ptr [rsp + 24]
	shl rcx, 2
	mov rsi, qword ptr [rdi]
	add rcx, rax
	cmp rcx, qword ptr [rsi]
	jne .LBB0_5
.LBB0_4:
	lea rcx, [rax + 4*rdx]
	mov qword ptr [rsi], rcx
.LBB0_5:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_6:
	mov eax, 4
	xor ecx, ecx
	xor edx, edx
	mov rsi, qword ptr [rdi]
	add rcx, rax
	cmp rcx, qword ptr [rsi]
	jne .LBB0_5
	jmp .LBB0_4
.LBB0_7:
	mov r14, rdi
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r12
	mov rdi, r14
	jmp .LBB0_0
.LBB0_8:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 16]
	mov rsi, qword ptr [rsp + 24]
	lea rdi, [rcx + 4*rdx]
	mov rdx, qword ptr [rsi]
	cmp rdi, qword ptr [rdx]
	jne .LBB0_9
	mov qword ptr [rdx], rcx
.LBB0_9:
	mov rdi, rax
	call _Unwind_Resume@PLT
